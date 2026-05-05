use std::ops::Deref;
use std::sync::{Arc, Mutex};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug)]
pub(crate) struct PoolClosed;

pub(crate) struct AsyncResourcePool<T> {
    inner: Arc<PoolInner<T>>,
}

impl<T> Clone for AsyncResourcePool<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

struct PoolInner<T> {
    idle: Mutex<Vec<T>>,
    permits: Arc<Semaphore>,
}

pub(crate) struct PooledResource<T> {
    value: Option<T>,
    inner: Arc<PoolInner<T>>,
    _permit: OwnedSemaphorePermit,
}

impl<T> AsyncResourcePool<T> {
    pub(crate) fn new(resources: Vec<T>) -> Self {
        let capacity = resources.len();
        assert!(capacity > 0, "async resource pool requires resources");
        Self {
            inner: Arc::new(PoolInner {
                idle: Mutex::new(resources),
                permits: Arc::new(Semaphore::new(capacity)),
            }),
        }
    }

    pub(crate) async fn acquire(&self) -> Result<PooledResource<T>, PoolClosed> {
        let permit = self
            .inner
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| PoolClosed)?;
        let value = self
            .inner
            .idle
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .pop()
            .ok_or(PoolClosed)?;
        Ok(PooledResource {
            value: Some(value),
            inner: self.inner.clone(),
            _permit: permit,
        })
    }
}

impl<T> Deref for PooledResource<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref().expect("pooled resource missing")
    }
}

impl<T> Drop for PooledResource<T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            self.inner
                .idle
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .push(value);
        }
    }
}
